import { Component, ElementRef, HostListener, QueryList, ViewChildren, OnInit, AfterViewInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { RouterLink } from '@angular/router';
import { AddHyphenPipe } from '../../../pipes/add-hyphen.pipe';
import { Subscription } from 'rxjs';

// ▲ SERVICES ▲
import { ArticleService } from '../../../services/navArticleObserver.service';
import { LanguageService } from '../../../services/navLanguage.service';
import { PageService } from '../../../services/navPage.service';
import { Language } from '../../../services/language.types';

// ▲ CONTENT ▲
import { content } from '../../content/content';
import { LightboxComponent } from '../../../features/lightbox/lightbox.component';

// Testing
import { HeaderT02Component } from '../header-t02/header-t02.component';

@Component({
  selector: 'doc-t02-mod-01-levels-structure',
  standalone: true,
  imports: [
    HeaderT02Component,
    CommonModule,
    RouterLink,
    LightboxComponent,
    AddHyphenPipe,
  ],
  templateUrl: './mod-01-levels-structure.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_Mod01_Component implements OnInit, AfterViewInit {
  // ▲ SERVICES ▲
  constructor(
    private languageService: LanguageService,
    private pageService: PageService,
    private intersectionService: ArticleService,
  ) { }

  // ▬▬▬ For inner content
  write: any;

  // ▬▬▬ Navigation Context
  currentLanguage: Language = '';
  currentTitle: string = '';
  currentPage: string = '';
  currentArticle: string = '';

  private languageSubscription: Subscription = new Subscription();

  // ▲ service Hash Sections
  @ViewChildren('section') sections!: QueryList<ElementRef>;

  // ████ OnInit ███
  ngOnInit(): void {
    this.subscribeToLanguageChanges();
    this.pageService.setCurrentPage('page_01');
    this.initializeContent(this.currentLanguage);
  }

  private initializeContent(language: Language): void {
    this.write = content[this.currentLanguage].title_02.page_01;
  }

  private subscribeToLanguageChanges(): void {
    this.languageSubscription = this.languageService.currentLanguage$.subscribe((language: string) => {
      this.currentLanguage = language as Language;
      this.initializeContent(this.currentLanguage); 
    });
  }

  // ███ Language Controller 
  private isValidLanguage(language: string): language is Language {
    return language === 'EN' || language === 'ES';
  }

  
  // ████ Hash Sections 
  hovered = false;

  showHash(event: Event) {
    this.hovered = true;
  }

  hideHash(event: Event) {
    this.hovered = false;
  }

  // ████ AfterViewInit ████
  ngAfterViewInit() {
    this.sections.forEach(section => {
      this.intersectionService.observe(section.nativeElement);
    });

    // Intersection Observer function
    this.intersectionService.getcurrentArticle().subscribe(id => {
      this.currentArticle = id;
    })

  }

}