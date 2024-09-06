import { AfterViewInit, Component, OnInit } from '@angular/core';
import { CommonModule, NgClass } from '@angular/common';
import { AddHyphenPipe } from '../../pipes/add-hyphen.pipe';

// ▲ SERVICES ▲
import { LanguageService } from '../../services/navLanguage.service';
import { IntersectionService } from '../../services/IntersectionObserver.service';
import { Language } from '../../services/language.types';

// ▲ CONTENT ▲
import { content } from '../../docs/content/content';
import { TitleService } from '../../services/navTitle.service';

@Component({
  selector: 'app-summary-bar',
  standalone: true,
  imports: [
    NgClass,
    CommonModule,
    AddHyphenPipe
  ],
  templateUrl: './summary-bar.component.html',
  styleUrl: './summary-bar.component.css'
})
export class SummaryBarComponent implements OnInit, AfterViewInit {
  // ▲ SERVICES ▲
  constructor(
    private intersectionService: IntersectionService,
    private languageService: LanguageService,
    private TitleService: TitleService
  ) { }

  // ▬▬▬ Navigation Context
  currentLanguage: Language = 'ES';
  currentTitle: string = '';
  // currentPage
  // currentArticle

  //  ▬▬▬ Intersection Section <section>
  idActive: string = '';

  // ████ AfterViewInit

  ngOnInit(): void {
    // Subscribe to Language Service
    this.languageService.currentLanguage$.subscribe((language: string) => {
      if (this.isValidLanguage(language)) {
        this.currentLanguage = language;
        this.getSummary();
      }
    });
    // Subsribe to Title Service
    this.TitleService.currentTitle$.subscribe(title => {
      this.currentTitle = title;
    });
  }

  ngAfterViewInit(): void {
    // Intersection Observer function
    this.intersectionService.getCurrentId().subscribe(id => {
      this.idActive = id;
    });
  }

  // ▬▬▬ Toogle Summary
  articleID: string[] = [];
  getPageSummary(id: string): void {
    
  }

  // ████ Map Tooogle Summary ███
  getSummary(): void {
    const pageContent = content[this.currentLanguage].title_02.page_01;
    this.articleID = Object.keys(pageContent).map((key) => pageContent[key].id);
  }

  // ███ Language Controller ███

  private isValidLanguage(language: string): language is Language {
    return language === 'EN' || language === 'ES';
  }


}
