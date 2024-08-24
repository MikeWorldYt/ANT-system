import { AfterViewInit, Component, OnInit } from '@angular/core';
import { CommonModule, NgClass } from '@angular/common';
import { AddHyphenPipe } from '../../pipes/add-hyphen.pipe';

// ▲ SERVICES ▲
import { LanguageService } from '../../services/lenguaje.service';
import { IntersectionService } from '../../services/IntersectionObserver.service';
import { Language } from '../../services/language.types';

// ▲ CONTENT ▲
import { content } from '../../docs/content/content';

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
    private languageService: LanguageService
  ) { }

  //  ▬▬▬ Intersection Section <section>
  idActive: string = '';

  // ████ AfterViewInit

  ngOnInit(): void {
    // Suscribe to Language Service
    this.languageService.language$.subscribe((language: string) => {
      if (this.isValidLanguage(language)) {
        this.currentLanguage = language;
        this.getSummary();
      }
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

  // ████ Map Tooogle Summary ███
  getSummary(): void {
    const moduleContent = content[this.currentLanguage].title_02.module_01;
    this.articleID = Object.keys(moduleContent).map((key) => moduleContent[key].id);
  }

  // ▬▬▬ Language
  currentLanguage: Language = 'ES';

  // ███ Language Controller ███

  private isValidLanguage(language: string): language is Language {
    return language === 'EN' || language === 'ES';
  }


}
